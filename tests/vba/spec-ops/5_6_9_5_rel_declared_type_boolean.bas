' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational declared type Boolean; Variant operand gives Variant result that may be Null
' EXPECT: Boolean
' EXPECT: Boolean
' EXPECT: Null
' EXPECT: Boolean
Public Sub Main()
    Dim a As Long, v As Variant
    a = 1
    Debug.Print TypeName(a = 1)
    Debug.Print TypeName("x" < "y")
    v = Null
    Debug.Print TypeName(a = v)
    v = 1
    Debug.Print TypeName(a = v)
End Sub
