' INV: vbal.6.1.2.7.1.12, func.typename
' SOURCE: spec ms-vbal 6.1.2.7.1.12 TypeName of the Missing data value is "Error"
' EXPECT: Error
Private Function Probe(Optional p As Variant) As String
    Probe = TypeName(p)
End Function

Public Sub Main()
    Debug.Print Probe()
End Sub
