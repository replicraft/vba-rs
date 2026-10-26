' INV: func.join
' SOURCE: derived Join concatenates array elements
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
