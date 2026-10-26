' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 single-letter spec, name value first character
' EXPECT: Double Double Empty
DefDbl m
Public Sub Main()
    Dim Mx, mY, nz
    Debug.Print TypeName(Mx) & " " & TypeName(mY) & " " & TypeName(nz)
End Sub
