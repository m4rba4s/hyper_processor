#include <stdio.h>

void __attribute__((constructor)) evil_init() { 
    printf("🔥 EVIL LIBRARY LOADED! 🔥\n"); 
}

void __attribute__((destructor)) evil_cleanup() {
    printf("💀 Evil library unloaded\n");
} 