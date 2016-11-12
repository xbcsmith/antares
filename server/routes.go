package server

import "net/http"

type Route struct {
	Name        string
	Method      string
	Pattern     string
	HandlerFunc http.HandlerFunc
}

type Routes []Route

var routes = Routes{
	Route{
		"Index",
		"GET",
		"/",
		Index,
	},
	Route{
		"AntarianIndex",
		"GET",
		"/antarians",
		AntarianIndex,
	},
	Route{
		"AntarianShow",
		"GET",
		"/antarians/{antarianId}",
		AntarianShow,
	},
    Route{
		"AntarianBuild",
		"GET",
		"/antarians/{antarianId}/build",
		AntarianBuild,
	},
	Route{
		"AntarianDownload",
		"GET",
		"/antarians/{antarianId}/download",
		AntarianDownload,
	},
	Route{
		"AntarianCreate",
		"POST",
		"/antarians",
		AntarianCreate,
	},
}
