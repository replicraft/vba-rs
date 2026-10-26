' INV: vbal.5.4.5.6, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.6 Line Input on a number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim s As String
    Line Input #4, s
End Sub
