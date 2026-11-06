#include "gem_vdiP.h"

/** loads the fonts for the device specified by \p handle and returns
 *  the number of additional fonts.
 *
 *  @param handle Device handle
 *  @param select 0 (reserved)
 *
 *  @return number of additional fonts
 *
 *  @since all VDI versions
 *
 *  Before you call vst_load_fonts(), call vq_gdos() or vq_vgdos() and make sure that the VDI can
 *  load additional fonts.
 *  
 */

short
vst_load_fonts (short handle, short select)
{
	short vdi_control[VDI_CNTRLMAX]; 
	short vdi_intout[1]; 
	
	VDI_PARAMS(vdi_control, &select, 0L, vdi_intout, vdi_dummy );
		
	/* NVDI expects a FONTHDR ptr here.
	 * NewTOS: the upstream binding nulls the fixed index 10, which is the chain
	 * only with 2-WORD pointers. The VDI reads the chain behind the scratch
	 * pointer and the offset word, each pointer N_PTRINTS WORDs wide
	 * (CONTRL[12..15] on LP64, CONTRL[10..11] on ILP32; docs/COMPATIBILITY.md
	 * "GDOS font chain"). A NULL chain answers 0 and changes nothing. */
	*__vdi_array_ptr(7 + N_PTRINTS + 1, vdi_control) = 0;

	VDI_TRAP (vdi_params, handle, 119, 0,1);
	
	return vdi_intout[0];
}
