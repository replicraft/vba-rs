' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, literals within the range of their declared type are valid
' EXPECT: Double
' EXPECT: Single
Public Sub Main()
    Debug.Print TypeName(1E308)
    Debug.Print TypeName(3.4E38!)
End Sub
