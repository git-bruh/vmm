#include <dirent.h>
#include <fcntl.h>
#include <poll.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <unistd.h>

int main(void) {
  char msg[4096] = "Hello from userspace!";
  size_t idx = 0;

  if (mount("dev", "/dev", "devtmpfs", 0, NULL) == -1) {
    return EXIT_FAILURE;
  }

  int kmsg = open("/dev/kmsg", O_WRONLY | O_APPEND);
  if (kmsg == -1) {
    return EXIT_FAILURE;
  }

  DIR *dir = opendir("/dev");
  if (!dir) {
    return EXIT_FAILURE;
  }

  // Write the original message once before overwriting it
  if (write(kmsg, msg, strlen(msg)) == -1) {
    return EXIT_FAILURE;
  }

  for (struct dirent *dp = NULL; (dp = readdir(dir)) != NULL;) {
    for (char *name = dp->d_name; *name != '\0'; name++) {
      msg[idx++] = *name;
    }

    msg[idx++] = ' ';
  }

  msg[idx++] = '\0';

  if (write(kmsg, msg, idx) == -1) {
    return EXIT_FAILURE;
  }

  closedir(dir);
  close(kmsg);
}