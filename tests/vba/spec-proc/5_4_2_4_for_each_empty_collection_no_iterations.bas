' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 For Each over a collection without elements completes immediately
' EXPECT: n=0
Public Sub Main()
    Dim c As New Collection
    Dim v As Variant, n As Long
    For Each v In c
        n = n + 1
    Next v
    Debug.Print "n=" & CStr(n)
End Sub
