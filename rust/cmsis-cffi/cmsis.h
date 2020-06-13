#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct UpdatePoll UpdatePoll;

typedef struct UpdateReturn UpdateReturn;

typedef struct {
  bool is_size;
  uintptr_t size;
} DownloadUpdate;

const char *err_get_last_message(void);

void err_last_message_free(char *ptr);

DownloadUpdate *update_pdsc_get_status(UpdatePoll *ptr);

UpdateReturn *update_pdsc_index_new(void);

bool update_pdsc_poll(UpdatePoll *ptr);

UpdateReturn *update_pdsc_result(UpdatePoll *ptr);
