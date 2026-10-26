' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, applies to ByVal arguments, function results and array elements
' EXPECT: arg=2
' EXPECT: arg=4
' EXPECT: ret=6
' EXPECT: ret=8
' EXPECT: elem=2
' EXPECT: elem=4
Sub ShowArg(ByVal n As Integer)
    Debug.Print "arg=" & CStr(n)
End Sub
Function Ret(ByVal d As Double) As Integer
    Ret = d
End Function
Public Sub Main()
    Dim a(1) As Long
    Dim d As Double
    ShowArg 2.5
    ShowArg 3.5
    Debug.Print "ret=" & CStr(Ret(6.5))
    Debug.Print "ret=" & CStr(Ret(7.5))
    d = 2.5: a(0) = d
    d = 3.5: a(1) = d
    Debug.Print "elem=" & CStr(a(0))
    Debug.Print "elem=" & CStr(a(1))
End Sub
