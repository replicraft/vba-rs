' INV: func.filter
' SOURCE: spec ms-vbal 6.1.2.11.1.7 Filter include and exclude
' EXPECT: banana
' EXPECT: mango
' EXPECT: apple
' EXPECT: cherry
Public Sub Main()
    Dim a As Variant, r() As String, i As Long
    a = Array("apple", "banana", "cherry", "mango")
    r = Filter(a, "an")
    For i = 0 To UBound(r): Debug.Print r(i): Next
    r = Filter(a, "an", False)
    For i = 0 To UBound(r): Debug.Print r(i): Next
End Sub
