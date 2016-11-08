package lib

import (
    "time"
    "fmt"
    "bytes"
    "encoding/json"
)

type Antarian struct {
	Id          string      `json:"id"`
	Name        string      `json:"name"`
	Version     string      `json:"version"`
	Release     string      `json:"release"`
	Uri         string      `json:"uri"`
	Running     bool        `json:"running"`
	Finished    bool        `json:"finished"`
	Start       time.Time   `json:"start"`
    End         time.Time   `json:"end"`
    BaseUrl     string      `json:"baseurl"`
    Requires    []string    `json:"requires"`
}

type Antarians []Antarian

func (a *Antarian) Filename() string {
    return fmt.Sprintf("%s-%s-%s.tgz", a.Name, a.Version, a.Release)
}

func (a *Antarian) UnmarshalJSON(raw []byte) error {

    var data struct {
        Name string
        Version string
        BaseUrl string
        Requires []string
    }

    r := bytes.NewReader(raw)
    if err := json.NewDecoder(r).Decode(&data); err != nil {
                    return fmt.Errorf("decode Data: %v", err)
                    }

    if a.Id == "" {
	    uuid, err := NewUUID()
	    if err != nil {
		    fmt.Printf("error: %v\n", err)
	    } else {
	    a.Id = uuid
        }
    }

    if a.Uri == "" {
        uri := GetUrl()
        a.Uri = uri
    }

    t := time.Now()
    a.Name = data.Name
    a.Version = data.Version
    a.Release = t.Format("20160101")
    a.BaseUrl = data.BaseUrl
    a.Requires = data.Requires
	a.Running = true
	a.Start = time.Now()
    return nil
}

func NewAntarian() *Antarian {
    uuid, err := NewUUID()
	if err != nil {
		fmt.Printf("error: %v\n", err)
	}
    return &Antarian{Id: uuid}
}
