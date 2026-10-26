' INV: vbal.5.4.5.1.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Print to a file number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Print #7, "x"
End Sub
