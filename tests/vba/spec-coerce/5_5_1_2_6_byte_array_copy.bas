' INV: vbal.5.5.1.2.6, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.6 Let-coercion to and from resizable Byte(), Byte() to Byte() is a copy
' EXPECT: orig=1,2,3
' EXPECT: copy=9,2,3
' EXPECT: ub=2
Public Sub Main()
    Dim a() As Byte
    Dim b() As Byte
    ReDim a(2)
    a(0) = 1: a(1) = 2: a(2) = 3
    b = a
    b(0) = 9
    Debug.Print "orig=" & CStr(a(0)) & "," & CStr(a(1)) & "," & CStr(a(2))
    Debug.Print "copy=" & CStr(b(0)) & "," & CStr(b(1)) & "," & CStr(b(2))
    Debug.Print "ub=" & CStr(UBound(b))
End Sub
