' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 start <= 0 is error 5
' ERROR: 5
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid(s, 0, 1) = "X"
End Sub
