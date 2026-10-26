' INV: vbal.5.4.5.6, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.6 Line Input takes a marked file number
' XFAIL: wrong not implemented: static checks of Line Input operands
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim s As String
    Open "/li.txt" For Input As #1
    Line Input 1, s
End Sub
