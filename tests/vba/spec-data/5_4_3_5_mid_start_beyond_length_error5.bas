' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 start greater than string length is error 5
' ERROR: 5
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid(s, 6, 1) = "X"
End Sub
