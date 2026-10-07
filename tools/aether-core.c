#define _GNU_SOURCE
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>

#define SOCKET_PATH "/run/aether-core.sock"
#define BUF_SIZE 256

static void write_all(int fd, const char *s) {
    size_t left = strlen(s);
    while (left) {
        ssize_t n = write(fd, s, left);
        if (n <= 0) return;
        s += n;
        left -= (size_t)n;
    }
}

static void handle(int fd) {
    char buf[BUF_SIZE];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    if (n <= 0) return;
    buf[n] = 0;
    if (!strncmp(buf, "PING", 4)) {
        write_all(fd, "AETHER_CORE_OK\n");
    } else if (!strncmp(buf, "STATE", 5)) {
        char line[128];
        FILE *f = fopen("/proc/uptime", "r");
        double up = 0.0;
        if (f) { fscanf(f, "%lf", &up); fclose(f); }
        snprintf(line, sizeof(line), "CORE=ONLINE UPTIME_SEC=%.0f PID=%ld\n", up, (long)getpid());
        write_all(fd, line);
    } else {
        write_all(fd, "ERR=UNKNOWN_COMMAND\n");
    }
}

int main(void) {
    unlink(SOCKET_PATH);
    int s = socket(AF_UNIX, SOCK_STREAM, 0);
    if (s < 0) return 1;
    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, SOCKET_PATH, sizeof(addr.sun_path) - 1);
    if (bind(s, (struct sockaddr *)&addr, sizeof(addr)) < 0) return 2;
    chmod(SOCKET_PATH, 0666);
    if (listen(s, 8) < 0) return 3;
    for (;;) {
        int c = accept(s, NULL, NULL);
        if (c >= 0) { handle(c); close(c); }
    }
}
