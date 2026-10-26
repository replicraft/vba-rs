' INV: vbal.6.1.2.8.1.11, func.iif
' SOURCE: spec ms-vbal 6.1.2.8.1.11 IIf returns the part selected by the expression
' EXPECT: yes
' EXPECT: no
Public Sub Main()
    Debug.Print IIf(1 < 2, "yes", "no")
    Debug.Print IIf(1 > 2, "yes", "no")
End Sub
