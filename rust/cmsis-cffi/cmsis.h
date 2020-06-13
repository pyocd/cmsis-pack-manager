#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <new>

struct UpdatePoll;

struct UpdateReturn;

struct DownloadUpdate {
  bool is_size;
  uintptr_t size;
};

extern "C" {

const char *err_get_last_message();

void err_last_message_free(char *ptr);

DownloadUpdate *update_pdsc_get_status(UpdatePoll *ptr);

UpdateReturn *update_pdsc_index_new();

bool update_pdsc_poll(UpdatePoll *ptr);

UpdateReturn *update_pdsc_result(UpdatePoll *ptr);

} // extern "C"
