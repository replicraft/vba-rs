' INV: vbal.6.1.2.7.1.6, func.ismissing
' SOURCE: spec ms-vbal 6.1.2.7.1.6 IsMissing true for omitted Optional Variant argument
' EXPECT: omitted=True
' EXPECT: given=False
' EXPECT: empty_given=False
Private Function Probe(Optional p As Variant) As Boolean
    Probe = IsMissing(p)
End Function

Public Sub Main()
    Debug.Print "omitted=" & CStr(Probe())
    Debug.Print "given=" & CStr(Probe(1))
    Debug.Print "empty_given=" & CStr(Probe(Empty))
End Sub
