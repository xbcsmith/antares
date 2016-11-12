package server

import (
	"fmt"
	"os"
	"time"
    "github.com/xbcsmith/antares/lib"
)

var antarians lib.Antarians

// Give us some seed data
func init() {
	h, _ := os.Hostname()
	currentUri := `http://` + h + `:8080`
	RepoCreateAntarian(lib.Antarian{Name: "AntarianMain", Uri: currentUri, Running: true, Start: time.Now()})
}

func RepoFindAntarian(id string) lib.Antarian {
	for _, s := range antarians {
		if s.Id == id {
			return s
		}
	}
	// return empty Antarian if not found
	return lib.Antarian{}
}

func RepoCreateAntarian(s lib.Antarian) lib.Antarian {
    uuid, err := lib.NewUUID()
    if err != nil {
        fmt.Printf("error: %v\n", err)
    }
	s.Id = uuid
	antarians = append(antarians, s)
	return s
}

func RepoDestroyAntarian(id string) error {
	for i, s := range antarians {
		if s.Id == id {
			antarians = append(antarians[:i], antarians[i+1:]...)
			return nil
		}
	}
	return fmt.Errorf("Could not find Antarian with id of %s to delete", id)
}
