' INV: vbal.5.4.5.6, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.6 target must be classified as a variable
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/li.txt" For Input As #1
    Line Input #1, "abc"
End Sub
