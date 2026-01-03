package main

import (
	"mbm/utils"
)

func main() {
	utils.Logger.Print("This is just a standard print")
	utils.Logger.Highlight("This is just a highlighted print")
	utils.Logger.Warning("This is just a warning print")
	utils.Logger.Error("This is just an error print")
	utils.Logger.Fatal("This is just a fatal print")
}
