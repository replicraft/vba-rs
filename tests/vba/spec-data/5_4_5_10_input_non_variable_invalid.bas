' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 input variable must be classified as a variable
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/i.txt" For Input As #1
    Input #1, 5
End Sub
