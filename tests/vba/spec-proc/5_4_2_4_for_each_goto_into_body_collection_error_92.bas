' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 jumping into a For Each body without evaluating the collection raises error 92 at Next
' EXPECT: in body
' ERROR: 92
Public Sub Main()
    Dim c As New Collection
    Dim v As Variant
    c.Add 1
    c.Add 2
    GoTo Body
    For Each v In c
Body:
        Debug.Print "in body"
    Next v
    Debug.Print "not reached"
End Sub
