' INV: vbal.6.1.2.11.1.7, func.filter
' SOURCE: spec ms-vbal 6.1.2.11.1.7 Filter errors when SourceArray is Null or not a one-dimensional array
' EXPECT: null=True
' EXPECT: scalar=True
' EXPECT: twodim=True
Public Sub Main()
    Dim v As Variant, r As Variant, m(1 To 2, 1 To 2) As String
    v = Null
    On Error Resume Next
    r = Filter(v, "a")
    Debug.Print "null=" & CStr(Err.Number <> 0)
    Err.Clear
    v = "abc"
    r = Filter(v, "a")
    Debug.Print "scalar=" & CStr(Err.Number <> 0)
    Err.Clear
    m(1, 1) = "a"
    v = m
    r = Filter(v, "a")
    Debug.Print "twodim=" & CStr(Err.Number <> 0)
End Sub
