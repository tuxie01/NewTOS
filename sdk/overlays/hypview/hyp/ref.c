/*
 * NewTOS patch of HypView hyp/ref.c (freemint/hypview @ 6f3eeeb),
 * patch site SDK04-S2 (TASK/SDK04; finding 6 of
 * docs/analysis/byte-order-survey-2026-10-29.md).
 *
 * Why (big-endian file format + LP64): a .REF file is m68k data -- "HREF",
 * then modules of {LONG module_len, LONG num_entries, entries}, ended by a
 * module header with num_entries 0; an entry is {BYTE type, BYTE size, name,
 * NUL, extra}, a label's line the big-endian WORD in its last two bytes. All
 * LONGs and WORDs are big-endian and 4/2 bytes wide. Upstream reads them
 * through C types: the magic into an `unsigned long` (8 bytes on LP64, four
 * file bytes in the low half, little-endian, the rest stack), REF_FILE's two
 * `long` fields (hyp.h, so `start` sits at byte 16 instead of 8), the
 * module step `((long *)pos)[1]` / `2*sizeof(long)`, and the line WORDs
 * through `short *`. Measured with the host harness
 * sdk/overlays/hypview/tests/ref-host.sh: upstream refuses every REF file,
 * the synthetic one and the FreeMiNT clone's mint-prg.ref / mint-de.ref
 * ("Illegal REF file. Magic value not found.", 18 of 19 checks fail).
 *
 * The patch decodes those fields from the bytes, big-endian, at their m68k
 * offsets (magic, num_entries at byte 4 of the loaded block, entries from
 * byte 8, the next module header, the two line WORDs), with the m68k signed
 * 32-bit LONG for the entry counts. The loaded block stays the raw file
 * image, as upstream keeps it; REF_FILE (hyp.h) is not used for field access
 * any more. Behaviour is upstream's byte for byte, including the alias
 * quirk (ref_findnode returns as an alias's line the WORD at its NUL, i.e.
 * the next entry's type byte). Every other line is upstream; changed places
 * are marked NEWTOS.
 */
/*
 * HypView - (c)      - 2006 Philipp Donze
 *               2006 -      Philipp Donze & Odd Skancke
 *
 * A replacement hypertext viewer
 *
 * This file is part of HypView.
 *
 * HypView is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * HypView is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with HypView; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 */

#ifdef __GNUC__
	#include <osbind.h>
#else
	#include <tos.h>
#endif
#include <string.h>
#include "../diallib.h"
#include "../defs.h"
#include "../hyp.h"


void RefGotoNode(DOCUMENT *doc, char *chapter,long node_num);

/* NEWTOS: on-disk offsets and big-endian decoding of the REF image */
#define REF_FILE_NUM_ENTRIES	4	/* LONG num_entries, after LONG module_len */
#define REF_FILE_START			8	/* first entry of the first module */
#define REF_MODULE_HEADER		8	/* {LONG module_len, LONG num_entries} */

static unsigned short
ref_be16(const char *p)
{
	const unsigned char *u = (const unsigned char *)p;
	return (unsigned short)((u[0] << 8) | u[1]);
}

static long
ref_be32(const char *p)	/* an m68k LONG: signed 32-bit */
{
	const unsigned char *u = (const unsigned char *)p;
	return (long)(int)(((unsigned int)u[0] << 24) | ((unsigned int)u[1] << 16) |
		((unsigned int)u[2] << 8) | (unsigned int)u[3]);
}

#define REF_START(ref)	((char *)(ref) + REF_FILE_START)	/* NEWTOS: was &ref->start */


REF_FILE *ref_load(short handle)
{
	REF_FILE *ref = NIL;
	long size,pos;
	char magic[4] = { 0 };	/* NEWTOS: was unsigned long */

	Fread(handle,4,magic);

	/*	"magischer Wert" im Datei-Kopf?	*/
	if(ref_be32(magic) != 0x48524546L) /*'HREF'*/	/* NEWTOS */
	{
		Debug("ERROR: Illegal REF file. Magic value not found.");
		return(NIL);
	}
	pos = Fseek(0, handle, 1);
	size = Fseek(0, handle, 2) - pos;
	Fseek(pos, handle, 0);
	
	/*	Speicher fuer Header und Indexdaten reservieren	*/
	ref = (REF_FILE *)Malloc(size);
	if(!ref)
	{
		Debug("Error while allocating %ld bytes for REF file.",size);
		return(NIL);
	}

	Fread(handle,size,ref);

	return(ref);
}


char *ref_find(REF_FILE *ref, short type)
{
char *ptr=REF_START(ref);	/* NEWTOS */
unsigned char size;
	if(ref == NIL)
		return(NULL);

	do
	{
		size=ptr[1];

/*
		Debug("%d (size %d) <%s>",*ptr,size,(size?ptr+2:""));
*/		if(*ptr==type)
			return(ptr+2);
		ptr+=size;
	}while(size);
	return(NULL);
}

short ref_findnode(REF_FILE *ref, char *string, long *node, short *line)
{
	char *ptr = REF_START(ref);	/* NEWTOS: was &ref->start */
	unsigned char size;
	long node_num=0;

	if(!ref || ref == NIL)
		return(FALSE);

	do
	{
		if (*ptr == REF_FILENAME)
			node_num=-1;
		else if(*ptr == REF_NODENAME)
			node_num++;
		size=ptr[1];
/*
		Debug("%d (size %d) <%s>",*ptr,size,(size?ptr+2:""));
*/		if(size)
		{
			if(strcmp(ptr+2,string)==0)
			{
				if(*ptr == REF_ALIASNAME)
					*line=(short)ref_be16(ptr+2+strlen(ptr+2));	/* NEWTOS */
				else
					*line=0;
				*node=node_num;
				return(TRUE);
			}
		}
		ptr+=size;
	}while(size);
	return(FALSE);
}

/*	Sucht in einer REF Datei nach dem Suchwort <string> und liefert
	eine verkettete Liste auf die passenden Nodes	*/
RESULT_ENTRY *ref_findall(REF_FILE *ref, char *string, short *num_results)
{
RESULT_ENTRY *list=NULL, *last_entry=NULL, prototype;
char *pos = REF_START(ref);	/* NEWTOS */
long ref_entries = ref_be32((char *)ref + REF_FILE_NUM_ENTRIES);	/* NEWTOS */
short found_one = FALSE;

	*num_results = 0;

	prototype.next=NULL;
	*prototype.path=0;
	*prototype.dbase_description=0;
	*prototype.node_name=0;
	prototype.line=0;

	do
	{
		while(ref_entries--) 
		{
		unsigned char size;
		
			size=pos[1];	
			switch(*pos)
			{
				case REF_FILENAME:
				{
					char *cptr;
					strcpy(prototype.path, pos+2);
			
					/*	Falls Datei-Erweiterung fehlt: hyp anfuegen	*/
					cptr=strrchr(prototype.path, '.');
					if(cptr == NULL)
						strcat(prototype.path, ".hyp" );
					break;
				}
				case REF_NODENAME:
					strcpy(prototype.node_name, pos+2);
					if(strcmp(string, pos+2) == 0) {
						found_one = TRUE;
						prototype.is_label=0;
					}
					break;
				case REF_ALIASNAME:
					if(strcmp(string, pos+2) == 0) {
						found_one = TRUE;
						prototype.is_label=0;
					}
					break;
				case REF_LABELNAME:
					if(strcmp(string, pos+2) == 0) {
						found_one = TRUE;
						prototype.is_label=1;
						prototype.line = ref_be16(pos+size-2);	/* NEWTOS */
					}					
					break;
				case REF_DATABASE:
					strcpy(prototype.dbase_description, pos+2);
					break;
				case REF_OS:
					break;
			}
			if(found_one)
			{
			RESULT_ENTRY *entry;
				entry=(RESULT_ENTRY *)Malloc(sizeof(RESULT_ENTRY));
				if(entry == NULL)
				{
					Debug("Out of Memory in ref_findall()");
					return list;
				}
				*entry = prototype;
				(*num_results)++;
				
				if(last_entry == NULL)
					list = entry;
				else
					last_entry->next = entry;
				last_entry = entry;
				found_one = FALSE;
			}
			pos=pos+size;
		}

		ref_entries = ref_be32(pos + 4);	/* NEWTOS: was ((long *)pos)[1] */
		pos+=REF_MODULE_HEADER;	/* NEWTOS: was 2*sizeof(long) */
	}while(ref_entries>0);
	
	return list;
}
