' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 string-argument may be Variant
' EXPECT: aXcde
Public Sub Main()
    Dim v As Variant
    v = "abcde"
    Mid(v, 2, 1) = "X"
    Debug.Print v
End Sub
