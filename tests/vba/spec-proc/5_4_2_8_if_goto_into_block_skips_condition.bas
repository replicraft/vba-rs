' INV: vbal.5.4.2.8, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.8 a jump into an If block does not evaluate the condition
' EXPECT: in-block
' EXPECT: cond-calls=0
Dim calls As Long
Function Cond() As Boolean
    calls = calls + 1
    Cond = False
End Function
Public Sub Main()
    GoTo Inside
    If Cond() Then
Inside:
        Debug.Print "in-block"
    End If
    Debug.Print "cond-calls=" & CStr(calls)
End Sub
