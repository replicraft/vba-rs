' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 each invocation has distinct ByVal parameter, local and result variables
' EXPECT: fact=120
' EXPECT: trace=1,2,3
Dim trace As String
Function Fact(ByVal n As Long) As Long
    Dim local As Long
    local = n
    If n <= 1 Then
        Fact = 1
    Else
        Fact = Fact(n - 1) * local
    End If
End Function
Sub Down(ByVal n As Long)
    Dim mine As Long
    mine = n
    If n > 1 Then Down n - 1
    If Len(trace) > 0 Then trace = trace & ","
    trace = trace & CStr(mine)
End Sub
Public Sub Main()
    Debug.Print "fact=" & CStr(Fact(5))
    Down 3
    Debug.Print "trace=" & trace
End Sub
