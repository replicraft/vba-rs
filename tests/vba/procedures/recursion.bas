' INV: stmt.function
' SOURCE: derived recursion
' EXPECT:  610 
Function Fib(n As Long) As Long
    If n < 2 Then Fib = n Else Fib = Fib(n - 1) + Fib(n - 2)
End Function
Public Sub Main()
    Debug.Print Fib(15)
End Sub
