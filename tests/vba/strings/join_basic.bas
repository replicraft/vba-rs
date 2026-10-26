' INV: func.join
' SOURCE: spec ms-vbal 6.1.2.11.1.16 Join delimiter default and empty
' EXPECT: a-b-c
' EXPECT: a b c
' EXPECT: abc
Public Sub Main()
    Dim a As Variant
    a = Array("a", "b", "c")
    Debug.Print Join(a, "-")
    Debug.Print Join(a)
    Debug.Print Join(a, "")
End Sub
