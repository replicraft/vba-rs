' INV: stmt.function
' SOURCE: spec ms-vbal 5.3.1 procedure invocation, 6.1.2.3.1.12 CStr of Boolean
' EXPECT: TrueTrueFalse
Function IsEven(n As Long) As Boolean
    If n = 0 Then IsEven = True Else IsEven = IsOdd(n - 1)
End Function
Function IsOdd(n As Long) As Boolean
    If n = 0 Then IsOdd = False Else IsOdd = IsEven(n - 1)
End Function
Public Sub Main()
    Debug.Print CStr(IsEven(10)) & CStr(IsOdd(7)) & CStr(IsEven(7))
End Sub
