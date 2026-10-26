' INV: stmt.open, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Print Statement error 52 for a file number that is not open
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Print #3, "x"
End Sub
