' INV: func.len, type.udt
' SOURCE: derived Len of a user-defined type sums its members: Integer (2) + Double (8) + three Bytes (3); LenB agrees (unverified for variable-length members)
' EXPECT:  13  13 
Type Inner
    A As Integer
    B As Double
End Type
Type Outer
    I As Inner
    Arr(1 To 3) As Byte
End Type
Public Sub Main()
    Dim o As Outer
    Debug.Print Len(o); LenB(o)
End Sub
