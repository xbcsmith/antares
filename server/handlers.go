package main

import (
	"encoding/json"
	"fmt"
	"github.com/gorilla/mux"
    "github.com/xbcsmith/antares/lib"
	"io"
	"io/ioutil"
	"net/http"
    "time"
)

func Index(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintln(w, "Antares!")
}

func AntarianIndex(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json; charset=UTF-8")
	w.WriteHeader(http.StatusOK)
	if err := json.NewEncoder(w).Encode(antarians); err != nil {
		panic(err)
	}
}

func AntarianShow(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	antarianId := vars["antarianId"]
    //fmt.Fprintln(w, "Antarian show:", antarianId)
    s := RepoFindAntarian(antarianId)
    w.Header().Set("Content-Type", "application/json; charset=UTF-8")
    w.WriteHeader(http.StatusOK)
    if err := json.NewEncoder(w).Encode(s); err != nil {
        panic(err)
    }
}

func AntarianBuild(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)

    type Build struct {
        Id      string      `json:"id"`
        Name    string      `json:"name"`
        Version string      `json:"version"`
        Start   time.Time   `json:"start"`
        Running bool        `json:"running"`
    }

    antarianId := vars["antarianId"]
    //fmt.Fprintln(w, "Antarian show:", antarianId)
    s := RepoFindAntarian(antarianId)

    build := &Build{s.Id,s.Name,s.Version,time.Now(),true}


    w.Header().Set("Content-Type", "application/json; charset=UTF-8")

    w.WriteHeader(http.StatusOK)
    if err := json.NewEncoder(w).Encode(build); err != nil {
        panic(err)
    }
}

func AntarianDownload(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	antarianId := vars["antarianId"]
    //fmt.Fprintln(w, "Antarian show:", antarianId)
    s := RepoFindAntarian(antarianId)

    type Download struct {
        Id      string      `json:"id"`
        Name    string      `json:"name"`
        Version string      `json:"version"`
        Url     string      `json:"url"`
    }

    dlurl := s.Uri + "/files/" + antarianId + "/" + s.Filename()
    download := &Download{s.Id, s.Name, s.Version, dlurl}
    w.Header().Set("Content-Type", "application/json; charset=UTF-8")
    w.WriteHeader(http.StatusOK)
    if err := json.NewEncoder(w).Encode(download); err != nil {
        panic(err)
    }
}

func AntarianCreate(w http.ResponseWriter, r *http.Request) {
	var antarian  lib.Antarian
	body, err := ioutil.ReadAll(io.LimitReader(r.Body, 1048576))
	if err != nil {
		panic(err)
	}
	if err := r.Body.Close(); err != nil {
		panic(err)
	}
	if err := json.Unmarshal(body, &antarian); err != nil {
		w.Header().Set("Content-Type", "application/json; charset=UTF-8")
		w.WriteHeader(422) // unprocessable entity
		if err := json.NewEncoder(w).Encode(err); err != nil {
			panic(err)
		}
	}

	s := RepoCreateAntarian(antarian)
	w.Header().Set("Content-Type", "application/json; charset=UTF-8")
	w.WriteHeader(http.StatusCreated)
	if err := json.NewEncoder(w).Encode(s); err != nil {
		panic(err)
	}
}
