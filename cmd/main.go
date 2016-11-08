package main

import (
	"encoding/json"
	"fmt"
    "os"
    "io/ioutil"
	"github.com/parnurzeal/gorequest"
    "github.com/xbcsmith/antares/lib"
)



func main() {
    antarian := lib.NewAntarian()

    raw, err := ioutil.ReadAll(os.Stdin)
    if err != nil {
        fmt.Println(err)
        os.Exit(-1)
    }
	if err := json.Unmarshal(raw, &antarian); err != nil {
		fmt.Println(err)
		return
	}
    fmt.Printf("%#v\n", antarian)
    response, err := json.Marshal(antarian)
    if err != nil {
        fmt.Println(err)
        return
    }
    url := lib.GetUrl()
	request := gorequest.New()
	resp, body, errs := request.Post(url).
		Set("Content-Type", "application/json; charset=UTF-8").
		Send(string(response)).
		End()
	if errs != nil {
		fmt.Println(errs)
		os.Exit(1)
	}
	fmt.Println("response Status:", resp.Status)
	fmt.Println("response Headers:", resp.Header)
	fmt.Println("response Body:", body)
}
