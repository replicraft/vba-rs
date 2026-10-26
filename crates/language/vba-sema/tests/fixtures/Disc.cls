VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "Disc"
Attribute VB_PredeclaredId = False
Option Explicit
Implements IShape

Public Event Resized(ByVal newRadius As Double)

Public Radius As Double
Private mName As String

Private Sub Class_Initialize()
    mName = "circle"
    Radius = 1
End Sub

Private Sub Class_Terminate()
    mName = vbNullString
End Sub

Public Property Get Label() As String
    Label = mName
End Property

Public Property Let Label(ByVal value As String)
    mName = value
End Property

Private Function IShape_Area() As Double
    IShape_Area = Geometry.PI * Radius * Radius
End Function

Private Property Get IShape_Name() As String
    IShape_Name = mName
End Property

Private Sub IShape_Rescale(ByVal factor As Double)
    Radius = Radius * factor
    RaiseEvent Resized(Radius)
End Sub
