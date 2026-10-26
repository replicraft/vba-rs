' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 string select expressions use the comparison operators (Option Compare Binary)
' EXPECT: b in a-c
' EXPECT: D not in a-c
Function F(ByVal s As String) As String
    Select Case s
        Case "a" To "c"
            F = s & " in a-c"
        Case Else
            F = s & " not in a-c"
    End Select
End Function
Public Sub Main()
    Debug.Print F("b")
    Debug.Print F("D")
End Sub
