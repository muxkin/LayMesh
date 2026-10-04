/* Isolate the measured child's RSS from the Python report writer's high-water
 * mark. No LayMesh/runtime dependency: this is an optional benchmark helper. */
#define _GNU_SOURCE
#include <errno.h>
#include <spawn.h>
#include <stdio.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
extern char **environ;

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    struct timespec start, end;
    struct rusage usage;
    posix_spawn_file_actions_t actions;
    posix_spawn_file_actions_init(&actions);
    posix_spawn_file_actions_adddup2(&actions, STDERR_FILENO, STDOUT_FILENO);
    clock_gettime(CLOCK_MONOTONIC, &start);
    pid_t pid;
    int result = posix_spawnp(&pid, argv[1], &actions, NULL, argv + 1, environ);
    posix_spawn_file_actions_destroy(&actions);
    if (result) { errno = result; perror("posix_spawnp"); return 2; }
    int status;
    while (wait4(pid, &status, 0, &usage) == -1) {
        if (errno != EINTR) { perror("wait4"); return 2; }
    }
    clock_gettime(CLOCK_MONOTONIC, &end);
    double seconds = end.tv_sec - start.tv_sec + (end.tv_nsec - start.tv_nsec) * 1e-9;
    long rss = usage.ru_maxrss;
#ifdef __APPLE__
    rss /= 1024;
#endif
    int code = WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
    printf("{\"seconds\":%.9f,\"peak_rss_kib\":%ld,\"exit_code\":%d}\n", seconds, rss, code);
    return code;
}
