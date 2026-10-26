' INV: vbal.5.6.2.1, expr.index
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, a function with no parameters is invoked with an empty list and a following argument list indexes the result
' XFAIL: wrong pending
' EXPECT: elem=20
' EXPECT: elem0=10
Function Arr() As Variant
    Arr = Array(10, 20, 30)
End Function
Public Sub Main()
    Debug.Print "elem=" & CStr(Arr(1))
    Debug.Print "elem0=" & CStr(Arr(0))
End Sub
