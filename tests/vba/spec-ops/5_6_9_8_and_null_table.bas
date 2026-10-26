' INV: vbal.5.6.9.8.2, op.and, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.2 And with Null operands
' EXPECT: Null
' EXPECT: Null
' EXPECT: 0
' EXPECT: 0
' EXPECT: Null
' EXPECT: Null
' EXPECT: Null
' EXPECT: Byte 0
' EXPECT: Boolean False
' EXPECT: Boolean True
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print TypeName(5 And n)
    Debug.Print TypeName(n And 5)
    Debug.Print CStr(0 And n)
    Debug.Print CStr(n And 0)
    Debug.Print TypeName(n And n)
    Debug.Print TypeName(True And n)
    Debug.Print TypeName(CByte(1) And n)
    Debug.Print TypeName(CByte(0) And n) & " " & (CByte(0) And n)
    Debug.Print TypeName(False And n) & " " & (False And n)
    Debug.Print TypeName(Not (n And False)) & " " & (Not (n And False))
End Sub
