VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "Bag"
Option Explicit
Private mItems As Collection
Private mKeys() As String
Private mCount As Long
Public Event Added(ByVal key As String, ByRef cancel As Boolean)
Public Event Removed(ByVal key As String)

Private Sub Class_Initialize()
    Set mItems = New Collection
    ReDim mKeys(1 To 4)
End Sub

Public Property Get Count() As Long
    Count = mItems.Count
End Property

Public Property Get Item(ByVal key As String) As Variant
Attribute Item.VB_UserMemId = 0
    If IsObject(mItems(key)) Then
        Set Item = mItems(key)
    Else
        Item = mItems(key)
    End If
End Property

Public Property Let Item(ByVal key As String, ByVal value As Variant)
    Add key, value
End Property

Public Sub Add(ByVal key As String, ByVal value As Variant)
    Dim cancel As Boolean
    RaiseEvent Added(key, cancel)
    If cancel Then Exit Sub
    mItems.Add value, key
    mCount = mCount + 1
    If mCount > UBound(mKeys) Then ReDim Preserve mKeys(1 To UBound(mKeys) * 2)
    mKeys(mCount) = key
End Sub

Public Function Keys() As Variant
    Dim r() As String, i As Long
    ReDim r(1 To mCount)
    For i = 1 To mCount
        r(i) = mKeys(i)
    Next
    Keys = r
End Function

Public Function NewEnum() As IUnknown
Attribute NewEnum.VB_UserMemId = -4
    Set NewEnum = mItems.[_NewEnum]
End Function
