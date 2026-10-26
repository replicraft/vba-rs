' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 a Variant counter is allowed
' EXPECT: 6 4
Public Sub Main()
    Dim v As Variant, s As Long
    For v = 1 To 3
        s = s + v
    Next v
    Debug.Print CStr(s) & " " & CStr(v)
End Sub
