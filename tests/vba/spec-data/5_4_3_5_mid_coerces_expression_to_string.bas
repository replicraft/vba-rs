' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 replacement is Let-coerced to String
' EXPECT: a12de
Public Sub Main()
    Dim s As String
    s = "abcde"
    Mid(s, 2, 2) = 12
    Debug.Print s
End Sub
