package utils

import (
	"fmt"
	"os"
	"time"
)

type logger struct{}
type Color int

const (
	Gray   Color = iota
	Purple Color = iota
	Yellow Color = iota
	Red    Color = iota
)

func getDate() string {
	return colorize(time.Now().Format(time.DateTime), Gray)
}

func (l logger) Print(content string) {
	fmt.Println(getDate(), content)
}

func (l logger) Highlight(content string) {
	fmt.Println(getDate(), colorize(content, Purple))
}

func (l logger) Warning(content string) {
	fmt.Println(getDate(), colorize(content, Yellow))
}

func (l logger) Error(content string) {
	fmt.Println(getDate(), colorize(content, Red))
}

func (l logger) Fatal(content string) {
	fmt.Println(getDate(), colorize(content, Red))
	os.Exit(1)
}

func colorize(content string, color Color) string {
	switch color {
	case Gray:
		return "\033[90m" + content + "\033[0m"
	case Purple:
		return "\033[35m" + content + "\033[0m"
	case Yellow:
		return "\033[33m" + content + "\033[0m"
	case Red:
		return "\033[31m" + content + "\033[0m"
	default:
		return content
	}
}

var Logger = logger{}
