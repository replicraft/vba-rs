' INV: vbal.5.2.3.6
' SOURCE: spec ms-vbal 5.2.3.6 circular dependency chains without const/enum/UDT/implements/event are allowed
' EXPECT: a3 b2 a1 b0
Public Sub Main()
    Debug.Print ModA.Ping(3)
End Sub
'#MODULE ModA standard
Public Function Ping(ByVal n As Long) As String
    Ping = "a" & CStr(n) & " " & ModB.Pong(n - 1)
End Function
'#MODULE ModB standard
Public Function Pong(ByVal n As Long) As String
    If n <= 0 Then
        Pong = "b" & CStr(n)
    Else
        Pong = "b" & CStr(n) & " " & ModA.Ping(n - 1)
    End If
End Function
