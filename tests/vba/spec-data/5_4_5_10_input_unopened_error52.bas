' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 Input from a number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim s As String
    Input #6, s
End Sub
