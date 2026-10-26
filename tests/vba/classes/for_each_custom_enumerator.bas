' INV: stmt.for-each-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/for-eachnext-statement For Each over an object collection uses the enumerator that its _NewEnum member (DISPID -4) returns
' EXPECT: a
' EXPECT: b
' EXPECT: c
' EXPECT: done
'#MODULE Bag class
Private mItems As Collection
Private Sub Class_Initialize()
    Set mItems = New Collection
End Sub
Public Sub Add(ByVal s As String)
    mItems.Add s
End Sub
Public Function NewEnum() As IUnknown
Attribute NewEnum.VB_UserMemId = -4
    Set NewEnum = mItems.[_NewEnum]
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Bag
    Dim x As Variant
    b.Add "a"
    b.Add "b"
    b.Add "c"
    For Each x In b
        Debug.Print x
    Next
    Debug.Print "done"
End Sub
