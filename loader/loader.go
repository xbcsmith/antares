package loader

import (
	"encoding/json"
	"fmt"
    "os"
    "net/http"
	"github.com/parnurzeal/gorequest"
    "github.com/xbcsmith/antares/lib"
)

type Loader struct {
    Response    string
    Status      string
    Header      http.Header
    Body        string
    Errors      []error
}

func Load(raw []byte) (*Loader, error) {

    antarian, err := lib.NewAntarian()
    if err != nil {
        fmt.Println(err)
        return &Loader{Errors: []error{err}}, nil
    }

    fmt.Printf("%#v\n", antarian)

    if err := json.Unmarshal(raw, &antarian); err != nil {
		fmt.Println(err)
        return &Loader{Errors: []error{err}}, nil
	}
    fmt.Printf("%#v\n", antarian)
    a, err := json.Marshal(antarian)
    if err != nil {
        fmt.Println(err)
        return &Loader{Errors: []error{err}}, nil
    }
    url := lib.GetUrl()
	request := gorequest.New()
    response := string(a)
	resp, body, errs := request.Post(url).
		Set("Content-Type", "application/json; charset=UTF-8").
		Send(response).
		End()
	if errs != nil {
		fmt.Println(errs)
		os.Exit(1)
	}
	fmt.Println("response Status:", resp.Status)
	fmt.Println("response Headers:", resp.Header)
	fmt.Println("response Body:", body)
    return &Loader{
        Response: response,
        Status: resp.Status,
        Header: resp.Header,
        Body:   body,
        Errors: errs,
    }, nil
}
