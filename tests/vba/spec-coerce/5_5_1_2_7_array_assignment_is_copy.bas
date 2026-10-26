' INV: vbal.5.5.1.2.7, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.7 Let-coercion to and from non-Byte arrays, same-element-type assignment makes a copy
' EXPECT: a=1,2,3
' EXPECT: b=1,99,3
' EXPECT: bounds=0,2
Public Sub Main()
    Dim a() As Long
    Dim b() As Long
    ReDim a(2)
    a(0) = 1: a(1) = 2: a(2) = 3
    b = a
    b(1) = 99
    Debug.Print "a=" & CStr(a(0)) & "," & CStr(a(1)) & "," & CStr(a(2))
    Debug.Print "b=" & CStr(b(0)) & "," & CStr(b(1)) & "," & CStr(b(2))
    Debug.Print "bounds=" & CStr(LBound(b)) & "," & CStr(UBound(b))
End Sub
