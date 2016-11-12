package server

import (
    "log"
    "net/http"
)

func Server() {
    router := NewRouter()
    log.Fatal(http.ListenAndServe(":8080", router))
}
