/*
 * cookie-host-intern.h -- stands in for cflib's intern.h when cookie-host.sh
 * compiles cookie.c on the host (TASK/SDK05). cookie.c takes nothing from
 * intern.h but TRUE/FALSE (the __PUREC__ branch, not compiled here) and the
 * GEM/osbind declarations it never calls; Getcookie() and C_FOUND come from
 * <mint/cookie.h>, which cookie.c includes itself.
 */
#ifndef COOKIE_HOST_INTERN_H
#define COOKIE_HOST_INTERN_H

#define TRUE  1
#define FALSE 0

#endif
