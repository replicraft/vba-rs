' INV: vbal.6.1.2.8.1.2, func.choose
' SOURCE: spec ms-vbal 6.1.2.8.1.2 Choose n-th element, Null when out of range
' EXPECT: 1=a
' EXPECT: 3=c
' EXPECT: 0=Null
' EXPECT: -1=Null
' EXPECT: 4=Null
Public Sub Main()
    Debug.Print "1=" & Choose(1, "a", "b", "c")
    Debug.Print "3=" & Choose(3, "a", "b", "c")
    Dim v As Variant
    v = Choose(0, "a", "b", "c")
    Debug.Print "0=" & IIf(IsNull(v), "Null", "x")
    v = Choose(-1, "a", "b", "c")
    Debug.Print "-1=" & IIf(IsNull(v), "Null", "x")
    v = Choose(4, "a", "b", "c")
    Debug.Print "4=" & IIf(IsNull(v), "Null", "x")
End Sub
