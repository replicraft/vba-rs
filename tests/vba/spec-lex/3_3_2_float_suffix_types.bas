' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, type suffix table
' EXPECT: Double
' EXPECT: Single
' EXPECT: Double
' EXPECT: Currency
' EXPECT: Single
' EXPECT: Double
' EXPECT: Currency
' EXPECT: Double
' EXPECT: Double
' EXPECT: Double
' EXPECT: Single
Public Sub Main()
    Debug.Print TypeName(1.5)
    Debug.Print TypeName(1.5!)
    Debug.Print TypeName(1.5#)
    Debug.Print TypeName(1.5@)
    Debug.Print TypeName(3!)
    Debug.Print TypeName(3#)
    Debug.Print TypeName(3@)
    Debug.Print TypeName(1E3)
    Debug.Print TypeName(.5)
    Debug.Print TypeName(1D3)
    Debug.Print TypeName(1E3!)
End Sub
