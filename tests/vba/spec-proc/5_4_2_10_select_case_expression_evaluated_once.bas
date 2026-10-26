' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 the select expression is evaluated once and reused for each clause
' EXPECT: calls=1 else
Dim calls As Long
Function Sel() As Long
    calls = calls + 1
    Sel = 99
End Function
Public Sub Main()
    Dim r As String
    Select Case Sel()
        Case 1
            r = "one"
        Case 2, 3
            r = "two-three"
        Case Is > 100
            r = "big"
        Case Else
            r = "else"
    End Select
    Debug.Print "calls=" & CStr(calls) & " " & r
End Sub
