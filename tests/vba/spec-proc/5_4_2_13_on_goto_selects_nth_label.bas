' INV: vbal.5.4.2.13
' SOURCE: spec ms-vbal 5.4.2.13 On n GoTo branches to the n-th label; out-of-range count falls through
' EXPECT: 1:one
' EXPECT: 2:two
' EXPECT: 3:three
' EXPECT: 0:fell
' EXPECT: 4:fell
Function Pick(ByVal n As Long) As String
    On n GoTo L1, L2, L3
    Pick = "fell"
    Exit Function
L1: Pick = "one": Exit Function
L2: Pick = "two": Exit Function
L3: Pick = "three"
End Function
Public Sub Main()
    Dim i As Variant
    For Each i In Array(1, 2, 3, 0, 4)
        Debug.Print CStr(i) & ":" & Pick(i)
    Next i
End Sub
