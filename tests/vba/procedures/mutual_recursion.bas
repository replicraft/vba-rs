' INV: stmt.function
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
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
