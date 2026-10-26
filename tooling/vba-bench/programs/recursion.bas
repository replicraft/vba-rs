Function Fib(ByVal n As Long) As Long
    If n < 2 Then
        Fib = n
    Else
        Fib = Fib(n - 1) + Fib(n - 2)
    End If
End Function
Sub Bump(ByRef x As Long, ByVal by As Long)
    x = x + by
End Sub
Public Sub Main()
    Dim i As Long, t As Long
    For i = 1 To 20000
        Bump t, i
    Next
    Debug.Print Fib(22); t
End Sub
