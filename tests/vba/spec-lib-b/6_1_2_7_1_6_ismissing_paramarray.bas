' INV: vbal.6.1.2.7.1.6, func.ismissing
' SOURCE: spec ms-vbal 6.1.2.7.1.6 IsMissing on a ParamArray argument returns False
' EXPECT: none=False
' EXPECT: some=False
Private Function Probe(ParamArray rest() As Variant) As Boolean
    Probe = IsMissing(rest)
End Function

Public Sub Main()
    Debug.Print "none=" & CStr(Probe())
    Debug.Print "some=" & CStr(Probe(1, 2))
End Sub
