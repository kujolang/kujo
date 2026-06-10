// Worker type docs
type Worker struct {}

// Public helper docs
func PublicHelper(value string) string {
    return value
}

func publicWithoutDocs(value string) string {
    return value
}

// Run method docs
func (w Worker) Run(value string) string {
    return value
}
