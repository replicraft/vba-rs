' INV: vbal.3.3.5
' SOURCE: spec ms-vbal 3.3.5 identifier tokens, digits and underscores are allowed after the first character
' EXPECT: 11 22 33
Public Sub Main()
    Dim a_1 As Long, B2c3 As Long, z__ As Long
    a_1 = 11: B2c3 = 22: z__ = 33
    Debug.Print a_1 & " " & b2c3 & " " & Z__
End Sub
