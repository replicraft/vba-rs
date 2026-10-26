' INV: vbal.6.1.2.7.1.4, func.isempty
' SOURCE: spec ms-vbal 6.1.2.7.1.4 IsEmpty true only for the Empty data value
' EXPECT: fresh=True
' EXPECT: empty=True
' EXPECT: zero=False
' EXPECT: emptystr=False
' EXPECT: null=False
' EXPECT: assigned=False
Public Sub Main()
    Dim v As Variant
    Debug.Print "fresh=" & CStr(IsEmpty(v))
    Debug.Print "empty=" & CStr(IsEmpty(Empty))
    Debug.Print "zero=" & CStr(IsEmpty(0))
    Debug.Print "emptystr=" & CStr(IsEmpty(""))
    Debug.Print "null=" & CStr(IsEmpty(Null))
    v = 1
    Debug.Print "assigned=" & CStr(IsEmpty(v))
End Sub
