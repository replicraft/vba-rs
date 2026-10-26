' INV: func.filter
' SOURCE: derived Filter selects matching elements
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
