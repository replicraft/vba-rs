' INV: vbal.5.2.3.1, vbal.5.2.3.1.1
' SOURCE: spec ms-vbal 5.2.3.1.1 variable list, per-variable as-clause
' EXPECT: Long String Empty Double
Private a As Long, b As String, c, d#
Public Sub Main()
    Debug.Print TypeName(a) & " " & TypeName(b) & " " & TypeName(c) & " " & TypeName(d)
End Sub
