#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <sys/io.h>
#include <pthread.h>
#include <stdbool.h>

#define EC_SC 0x66   // Command register
#define EC_DATA 0x62 // Data register

void ec_wait_ibf() {
    while (inb(EC_SC) & 0x02);  // Wait for IBF to be 0
}

void ec_wait_obf() {
    while (!(inb(EC_SC) & 0x01));  // Wait for OBF to be 1
}

void ec_init() {
    // Initialize permissions for I/O operations
    if (ioperm(EC_SC, 1, 1) || ioperm(EC_DATA, 1, 1)) {
        perror("ioperm");
        exit(1);
    }
}

uint8_t ec_read(uint8_t addr) {
    ec_wait_ibf();
    outb(0x80, EC_SC);  // Command: Read
    ec_wait_ibf();
    outb(addr, EC_DATA);
    ec_wait_obf();
    return inb(EC_DATA);
}

void ec_write(uint8_t addr, uint8_t value) {
    ec_wait_ibf();
    outb(0x81, EC_SC);  // Command: Write
    ec_wait_ibf();
    outb(addr, EC_DATA);
    ec_wait_ibf();
    outb(value, EC_DATA);
}

void ec_dump_selected() {
    printf("Dumping SSRM region (0x50-0x59)...\n");
    uint8_t registers[] = {0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59};
    for (int i = 0; i < sizeof(registers)/sizeof(registers[0]); i++) {
        uint8_t value = ec_read(registers[i]);
        printf("Register 0x%02X: 0x%02X\n", registers[i], value);
        // Interpret specific registers based on ACPI table information
        switch (registers[i]) {
            case 0x50:
                printf("Temperature: %d\n", value);
                break;
            case 0x51:
                printf("Fan Speed: %d\n", value);
                break;
            case 0x52:
                printf("Flag: %d\n", value & 0x01);
                break;
            // Add more cases as needed based on ACPI analysis
            default:
                break;
        }
    }
}

void *ec_monitor_loop(void *arg) {
    printf("Starting EC monitor loop...\n");
    while (true) {
        // Example: Monitor temperature register
        uint8_t temp = ec_read(0x00); // Assuming 0x00 is the temperature register
        if (temp > 75) { // Example threshold
            printf("Warning: High temperature detected: %d\n", temp);
        }
        sleep(1); // Monitor every second
    }
    return NULL;
}

void acpi_check() {
    printf("Checking ACPI flags...\n");
    // Example: Check a specific ACPI flag
    uint8_t acpi_flag = ec_read(0x01); // Assuming 0x01 is an ACPI flag register
    printf("ACPI Flag: 0x%02X\n", acpi_flag);
}

int main() {
    ec_init();
    ec_dump_selected();
    
    pthread_t monitor_thread;
    pthread_create(&monitor_thread, NULL, ec_monitor_loop, NULL);
    
    acpi_check();
    
    pthread_join(monitor_thread, NULL);
    return 0;
} 