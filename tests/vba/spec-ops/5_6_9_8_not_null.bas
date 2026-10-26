' INV: vbal.5.6.9.8.1, op.not, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.1 Not Null is Null
' EXPECT: True
' EXPECT: Null
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print IsNull(Not n)
    Debug.Print TypeName(Not n)
End Sub
