' INV: vbal.5.5.1.2.7, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.7 Let-coercion to and from non-Byte arrays, an array value may be Let-coerced to Variant and back to a same-typed array
' EXPECT: isarray=True
' EXPECT: back=7,8
Public Sub Main()
    Dim a() As Long
    Dim b() As Long
    Dim v As Variant
    ReDim a(1)
    a(0) = 7: a(1) = 8
    v = a
    Debug.Print "isarray=" & CStr(IsArray(v))
    b = v
    Debug.Print "back=" & CStr(b(0)) & "," & CStr(b(1))
End Sub
